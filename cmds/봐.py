# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *
from lib.func import *

class CmdObj(Command):

    def view(self, obj, ob):
        p = int(obj['보관수량'])
        pm = obj['보관증가은전']
        pp = obj['보관최대수량']
        
        ref = '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        buf = '◁ %s의 %s ▷' % (obj['주인'], obj['이름'])
        buf = fillSpace(ref, buf)
        ob.sendLine('[1m[44m[37m%s[0m[40m[37m' % buf)
        ob.sendLine('───────────────────────────────────────')
        c = 0
        cnt = 0
        msg = ''
        for item in obj.objs:
            c += 1
            if item.isOneItem():
                s = '[1;36m' + stripANSI(item['이름']) + '[0;37m' + ' ' + item.getOptionStr()
            else:
                s = item['이름'] + ' ' + item.getOptionStr()
            s = '[%4d] %s' % (c, s)
            s1 = stripANSI(s)
            #m = '%-38s' % (s + space)
            m = '%s' % fillSpace(38, s)
            if cnt == 1 and len(stripANSI(m).encode('euc-kr')) > 38:
                msg += '\r\n'
            msg += m
            if len(stripANSI(s).encode('euc-kr')) > 38:
                msg += '\r\n'
                cnt = 0 
            else:
                cnt += 1
            #msg += '·%-24s' % (s + space)
            #msg += '[1;36m·[0;36m%-38s[0;37m  ' % (item['이름'] + ' ' + item.getOptionStr())
            if cnt == 2:
                msg += '\r\n'
                cnt = 0
        if msg != '':
            if msg[-1] == '\n':
                msg = msg[:-2]
            ob.sendLine(msg)

        if c == 0:
            ob.sendLine('☞ 아무것도 없습니다.')

        if obj['보관수량'] == obj['보관최대수량']:
            buf = '◆ 수량 (%d/%d)' % ( len(obj.objs), obj['보관수량'])
        else:
            buf = '◆ 수량 (%d/%d)  ◆ 최대수량 (%d)  ◆ 확장에 필요한 은전 (%d/%d)' % ( len(obj.objs), obj['보관수량'], \
            obj['보관최대수량'], getInt(obj['은전']), obj['보관증가은전'])
        ob.sendLine('───────────────────────────────────────')
        buf = fillSpace(ref, buf)
        ob.sendLine('[0m[47m[30m%s[0m[40m[37m' % buf)
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
#보관함 정렬 수정
    def viewBox(self, obj, ob):
        p = int(obj['보관수량'])
        pm = obj['보관증가은전']
        pp = obj['보관최대수량']
        
        ref = '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        buf = '◁ %s의 %s ▷' % (obj['주인'], obj['이름'])
        buf = fillSpace(ref, buf)
        ob.sendLine('[1m[44m[37m%s[0m[40m[37m' % buf)
        ob.sendLine('───────────────────────────────────────')
        c = 0
        nCnt = {}
        for item in obj.objs:
            c += 1
            nc = 0
            try:
                nc = nCnt[item['이름']]
            except:
                nCnt[item['이름']] = 0
            nCnt[item['이름']] = nc + 1
        if c == 0:
            ob.sendLine('☞ 아무것도 없습니다.')
        else:
            cnt = 0
            msg = ''
            c = 0
            for name in nCnt:
                if len(name) != len(stripANSI(name)):
                    a = '[0;36m'
                else:
                    a = ''
                nc = nCnt[name]
                if nc == 1:
                    buf = '[1;36m·[0;36m%s[0;37m' % name
                else:
                    buf = '[1;36m·[0;36m%s %s%d개[0;37m' % (name, a, nc)
                c += 1
                #보관함 정렬 수정
                #m = '%-22s' % (buf + space)
                m = '%s' % fillSpace(22, buf)
                if cnt == 1 and len(stripANSI(m).encode('euc-kr')) > 22:
                    msg += '\r\n'
                msg += m
                if len(stripANSI(buf).encode('euc-kr')) > 22:
                    msg += '\r\n'
                    cnt = 0 
                else:
                    cnt += 1
                if cnt == 3:
                    msg += '\r\n'
                    cnt = 0
                #msg += '[1;36m·[0;36m%-20s[0;37m  ' % (buf+space)
                #msg += '[1;36m·[0;36m%-20s[0;37m  ' % buf
                #if c % 3 == 0:
                    #msg += '\r\n'
            if c % 3 == 0:
                msg = msg[:-2]
            ob.sendLine(msg)
        if obj['보관수량'] == obj['보관최대수량']:
            buf = '◆ 수량 (%d/%d)' % ( len(obj.objs), obj['보관수량'])
        else:
            buf = '◆ 수량 (%d/%d)  ◆ 최대수량 (%d)  ◆ 확장에 필요한 은전 (%d/%d)' % ( len(obj.objs), obj['보관수량'], \
            obj['보관최대수량'], getInt(obj['은전']), obj['보관증가은전'])
        ob.sendLine('───────────────────────────────────────')
        buf = fillSpace(ref, buf)
        ob.sendLine('[0m[47m[30m%s[0m[40m[37m' % buf)
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

#관리자용 정보 추가
    def infoPlayer(self, obj, ob):
        if len(obj.target) !=0:
            cnt=1
            for target in obj.target:
                ob.sendLine(' [%02d 목표] %s ' % (cnt, target['이름']))
                cnt += 1
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

    def infoMob(self, obj, ob):
        ob.sendLine('│ [레  벨] %15s  │ [상  태] %15s  │' % (getInt(obj['레벨']), obj.act))
        temp1 = '%d/%d' % (obj.getHp(), obj.getMaxHp())
        temp2 = '%d/%d' % (obj.getMp(), obj.getMaxMp())
        ob.sendLine('│ [체  력] %15s  │ [내  공] %15s  │ ' % (temp1, temp2))
        ob.sendLine('│ [맷  집] %15d  │ [민  첩] %15d  │' % (obj.getArm(), obj.getDex()))
        temp1 = '----------'
        if obj.skill != None:
            temp1 = obj.skill.name
        ob.sendLine('│ [  힘  ] %15s  │ [스  킬] %15s  │' % ( obj.getStr(), temp1))
        if getInt(obj['난이도']) >=1:
            ob.sendLine('│ [命  中] %15d  │ [回  避] %15d  │' % (obj.getHit(), obj.getMiss()))
            ob.sendLine('│ [必  殺] %15d  │ [  運  ] %15d  │' % (obj.getCritical(), obj.getCriticalChance()))
        if len(obj.target) !=0:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('★ 목표 대상')
            cnt =1
            msg =''
            for target in obj.target:
                msg += ' [%02d] %-10s    ' % (cnt, target['이름'])
                if (cnt % 3) == 0:
                    msg += '\r\n'
                cnt += 1
            ob.sendLine(msg)
        if len(obj.skillList) != 0:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('★ 공격 스킬 목록')
            cnt =1
            msg =''
            for skill in obj.skillList:
                msg += ' [%02d] %-10s ' % (cnt,skill[0].name)
                if (cnt % 3) == 0:
                    msg += '\r\n'
                cnt += 1
            ob.sendLine(msg)
        if len(obj.defskillList) != 0:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('★ 기타 스킬 목록')
            cnt =1
            msg =''
            for skill in obj.defskillList:
                msg += ' [%02d] %-10s ' % (cnt,skill[0].name)
                if (cnt % 3) == 0:
                    msg += '\r\n'
                cnt += 1
            ob.sendLine(msg)
        if len(obj.skills) != 0:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('★ 무공집결상태')
            for s in obj.skills:
                inc = 1
                if s.name in obj.skillMap:
                    inc = obj.skillMap[s.name][0]
                #n = s['방어시간'] + s['방어시간증가치'] * (inc - 1)
                #t = s.start_time
                n = s.end_time
                t = time.time()
                t2 = int(n - t)
                cnt = len(obj.strBar)
                a = int(t * 10 // n)
                if a < 0:
                    a = 0
                if a >= cnt:
                    a = cnt - 1
                buf = '%5dː%s' % (t2, obj.strBar[a])
                ob.sendLine('[1m[40m[36m·[0m[40m[37m%-14s│%-12s│ %s' % (s.name, s['방어상태출력'], buf)) 
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        
    def cmd(self, ob, line):
        if len(line) == 0:
            ob.viewMapData()
            return
        if ob.env == None:
            print(ob['이름'])
            return

        words = line.split()
        if line == '호위' or (len(words) > 1 and words[1] == '호위'):
            ob.do_command(line, True)
            return
        name, order = getNameOrder(line)

        
        if line == '나':
            obj = ob
        else:
            obj = ob.findObjInven(name, order)

        if obj == None:
            obj = ob.env.findObjName(line)
            if obj == None:
                ob.sendLine('☞ 당신의 안광으로는 그런것을 볼수 없다네')
                return
        if getInt(ob['관리자등급']) >= 1000 and is_player(obj) == False:
            ob.sendLine('Index : %s' % obj.index)
        if (line == '무기고' or line == '화초장' or line == '한옥장'or line == '진열장') and is_box(obj):
            self.view(obj, ob)
#보관함 정렬 수정
        elif is_box(obj):
            self.viewBox(obj, ob)
        else:
            obj.view(ob)
            if getInt(ob['관리자등급']) >= 1000:
                if is_player(obj):
                    self.infoPlayer(obj, ob)
                elif is_mob(obj):
                    self.infoMob(obj, ob)
        if is_player(obj) and obj != ob:
            obj.sendLine('\r\n%s 당신을 살펴봅니다.' % ob.han_iga())
            obj.lpPrompt()

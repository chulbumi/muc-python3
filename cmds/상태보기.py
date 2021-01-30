# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):

    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 운영자 명령: [대상] 상태보기')
            return
        obj = ob.env.findObjName(line)
        if obj == None or is_item(obj):
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if is_player(obj) == False:
            ob.sendLine('Index : %s' % obj.index)
        write = ob.sendLine
        get = obj.get
        write('┏━━━━━━━━━━━━━━━━━━━━━━━━━┑')
        write('│[0m[44m[1m[37m ▷▶▷▶▷ %10s의 현재 상태     ◁◀◁◀◁ [0m[40m[37m│' % obj['이름'])
        write('┝━━━━━━━━━━━━┯━━━━━━━━━━━━┥')
        write('│ [레  벨]       [%5d] │ [나  이]          %4d │' % (get('레벨'), getInt(get('나이'))) )
        if is_player(obj):
            temp = '%d/%d' % (obj.getHp(), obj.getMaxHp())
        else:
            temp = '%d/%d' % (obj.hp, get('체력'))
        tmp = get('성격')
        if tmp == '':
            tmp = '--------'
        write('│ [체  력] %13s │ [성  격]      %8s │' % (temp, tmp))
        temp = 0
        tmp = get('성별')
        if tmp == '':
            tmp = '--'
        write('│ [  힘  ]  %4d + %5d │ [성  별]            %2s │' % (obj.getAttPower(), obj.getStr(), tmp) )

        tmp = get('소속')
        if tmp == '':
            tmp = '--------'
        write('│ [맷  집] %5d + %5d │ [소  속]      %8s │' % (obj.getArmor(), obj.getArm(), tmp) )
        tmp = get('직위')
        if tmp == '':
            tmp = '--------'
        write('│ [민  첩]  %12d │ [직  위]      %8s │' % (obj.getDex(), tmp) )
        tmp = get('배우자')
        if tmp == '':
            tmp = '--------'
        temp = '%d/%d' % (obj.getMp(), obj.getMaxMp())
        write('│ [내  공]  %12s │ [배우자]      %8s │' % (temp, tmp) )

        temp = '%d/%d' % (obj.getItemWeight(), obj.getStr() * 10)
        
        write('│ [현  경]  %12d │ [소지품]  %12s │' % (getInt(obj['현재경험치']), temp) )

        write('│ [목  경]  %12d │ [분  노]           %3d │' % (obj.getTotalExp(), 0) )
        write('│ [命  中] %15d │ [回  避] %15d │' % (obj.getHit(), obj.getMiss()))
        write('│ [必  殺] %15d │ [  運  ] %15d │' % (obj.getCritical(), obj.getCriticalChance()))
        write('├────────────┴────────────┤')
        write('│[0m[47m[30m [은  전]                    %20d [0m[40m[37m│' % getInt(get('은전')))
        write('┕━━━━━━━━━━━━━━━━━━━━━━━━━┙')
        from lib.script import get_hp_script, get_mp_script
        #eyehint님 수정
        write( '★ ' + postPosition1(get('이름') + get_hp_script(ob)) )
        #write( '★ ' + han_parse(get('이름'), get_hp_script(ob)) )
        p = obj.getInsureCount()
        if p == 0:
            ob.sendLine('★ %s의 표국보험은 효력이 없습니다.' % obj.getNameA())
        else:
            ob.sendLine('★ %s %d번의 표국보험 혜택을 받으실 수 있습니다.' % (obj.han_iga(), p))
        #eyehint님 수정
        write( '★ ' + postPosition1(get('이름') + get_mp_script(obj)) )
        #write( '★ ' + han_parse(get('이름'), get_mp_script(obj)) )

        p = getInt(obj['특성치'])
        if p != 0:
            ob.sendLine('★ %s %d개의 여유 특성치를 보유하고 있습니다.' % (obj.han_un(), p))

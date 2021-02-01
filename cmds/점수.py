# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.func import fillSpace
from include.ansi import *

class CmdObj(Command):

    def cmd(self, ob, line):
        write = ob.sendLine
        get = ob.get
        write('☞ ' + ob.getDesc(True))
        write('┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┑')
        write('│[0m[44m[1m[37m ▷▶▷▶▷▶      당신의 현재 상태      ◀◁◀◁◀◁ [0m[40m[37m│')
        write('┝━━━━━━━━━━━━━┯━━━━━━━━━━━━━┥')
        write('│ [레  벨]        [%6d] │ [나  이]          %6d │' % (get('레벨'), get('나이')) )
        temp = '%d/%d' % (ob.getHp(), ob.getMaxHp())
        tmp = get('성격')
        if tmp == '':
            tmp = '----------'
        write('│ [체  력] %15s │ [성  격] %s │' % (temp, fillSpace(15, tmp, True)))
        temp = 0
        
        write('│ [  힘  ]  %5d + %6d │ [성  별] %s │' % (ob.getAttPower(), ob.getStr(), fillSpace(15, get('성별'), True)) )
        tmp = get('소속')
        if tmp == '':
            tmp = '----------'
        write('│ [맷  집] %6d + %6d │ [소  속] %s │' % (ob.getArmor(), ob.getArm(), fillSpace(15, tmp, True)) )
        tmp = get('직위')
        if tmp == '':
            tmp = '----------'
        else:
            g = GUILD[ob['소속']]
            if '%s명칭' % ob['직위'] in g:
                tmp = g['%s명칭' % ob['직위']]
            else:
                tmp = ob['직위']
        write('│ [민  첩] %15d │ [직  위] %s │' % (ob.getDex(), fillSpace(15, tmp, True)) )
        write('│ [命  中] %15d │ [回  避] %15d │' % (ob.getHit(), ob.getMiss()))
        write('│ [必  殺] %15d │ [  運  ] %15d │' % (ob.getCritical(), ob.getCriticalChance()))
        tmp = get('배우자')
        if tmp == '':
            tmp = '----------'
        temp = '%d/%d' % (ob.getMp(), ob.getMaxMp())
        #write('│ [내  공] %15d │ [배우자]      %10s │' % (ob.getMp(), tmp) )
        write('│ [내  공] %15s │ [배우자] %s │' % (temp, fillSpace(15, tmp, True)) )

        temp = '%d/%d' % (ob.getItemWeight(), ob.getStr() * 10)
        write('│ [현  경] %15d │ [소지품] %15s │' % (ob['현재경험치'], temp) )
        anger = getInt(ob['분노'])
        if anger >= 100:
            temp = '[1;31m%d[0;37m' % anger
        else:
            temp = '%d' % anger
        write('│ [목  경] %15d │ [분  노]             %3s │' % (ob.getTotalExp(), temp))
        write('├─────────────┴─────────────┤')
        write('│[0m[47m[30m [은  전]    %40d [0m[40m[37m│' % get('은전'))
        if ob['금전'] == '':
            ob['금전'] = 0
        if ob['금전'] > 0:
            write('│[0m[43m[30m [금  전]    %40d [0m[40m[37m│' % get('금전'))
        write('┕━━━━━━━━━━━━━━━━━━━━━━━━━━━┙')
        if ob['소속'] != '':
            g = GUILD[ob['소속']]
            if '%s명칭' % ob['직위'] in g:
                buf = g['%s명칭' % ob['직위']]
            else:
                buf = ob['직위']
            temp = ''
            if ob['방파별호'] != '':
                temp = '(%s)' % ob['방파별호']
            write('★ %s%s [1m【%s】[0m 문파의 [1m%s%s[0m 입니다.' % \
                ('당신', han_un('당신'), ob['소속'], buf, temp ))
        from lib.script import get_hp_script, get_mp_script
        write('★ ' + postPosition1('당신' + get_hp_script(ob)))
        #write( '★ ' + han_parse('당신', get_hp_script(ob)) )
        p = ob.getInsureCount()
        if p == 0:
            ob.sendLine('★ 당신의 표국보험은 효력이 없습니다.')
        else:
            ob.sendLine('★ 당신은 %d번의 표국보험 혜택을 받으실 수 있습니다.' % p)
        write('★ ' + postPosition1('당신' + get_mp_script(ob)))
        #write( '★ ' + han_parse('당신', get_mp_script(ob)) )

        p = getInt(ob['특성치'])
        if p != 0:
            ob.sendLine('★ 당신은 %d개의 여유 특성치를 보유하고 있습니다.' % p)
